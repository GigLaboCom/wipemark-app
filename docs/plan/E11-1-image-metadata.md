# E11-1 — Provenance metadata in PNG, JPEG and WebP (library only)

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E11, images (phase 2)                                                                                              |
| Spec scopes      | OV §9 (containers, AI flags, knobs), OV §11 (the gate: the decoded raster's sha256 is identical before and after)                     |
| Task             | Watchword FILE `wipemark-task-e11-1-image-metadata-2026-10-04`                                                                         |
| Depends on       | E0's vocabulary in `crates/wipemark-image/src/lib.rs`; `wipemark_core::report::not_established` (E1)                                   |
| Unblocks         | E11-2 (TIFF, HEIC/AVIF), and the surfaces — CLI, MCP, the windows — in a later step                                                    |
| Files touched    | `crates/wipemark-image/**`, `fixtures/image/**`, `fixtures/README.md`, `.gitattributes`, `Cargo.lock`, `docs/architecture/images.md`, this document, its report |
| Size             | ~2 days for one agent; pure code — no GPU, no model, no window                                                                       |

## §0 Ground rules

### 0.1 Start here

Read `CLAUDE.md` in full — if it and this document disagree, `CLAUDE.md`
wins and the report says so. Branch `e11/image-metadata` from
`feat/e0-e6-shell`; one commit,
`E11-1: Image metadata — PNG, JPEG, WebP, pixels never re-encoded`;
push the branch only, never `main`, no PR. Leave
` m vendor/gpui-component` unstaged.

### 0.2 Where code goes

```
core ← image          (scripts/check-dep-direction.sh: "wipemark-image": {"wipemark-core"})
```

- Everything is `crates/wipemark-image`. No surface: no CLI command, no
  MCP tool, no window, no queue — the CLI and the app are being changed
  on other branches.
- **Only applications localize.** The crate returns values — kinds,
  signals, generators, defects — and never a sentence a person reads.
  The `Display` of an error is for a log line.
- **The bytes decide what a thing is**, and `wipemark-intake` decides it
  for the product. This crate is handed bytes already known to be an
  image; it checks the one signature its own parser needs anyway and
  refuses anything else as `UnknownContainer`. It does not depend on
  `wipemark-intake` (the dependency rule forbids it, and the recogniser
  is not what is needed — the parser is).

### 0.3 Rules of this repository that bind this document

- **The third shelf is never empty.** Every `ImageReport` and every
  `StripReport` carries `not_established`, and it always contains
  `unknown-mark-schemes`: the pixel domain (SynthID-class marks) is a
  scheme this build does not implement. Nothing says "undetectable" and
  "no AI metadata found" is about **metadata**, never the picture.
- **Tests must be able to fail.** Every protection below has a mutation
  that turns a named test red; the report records each one.
- **Pixels are never re-encoded.** Nothing in the crate decodes a pixel;
  decoders are dev-dependencies, used only by the gate.

### 0.4 Gates — all green before the branch is pushed

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

The lock moved once, with `cargo check -p wipemark-image` (edges only:
every package added is already in the tree through gpui).

---

## §1 Goal

`wipemark-image` opens a PNG, a JPEG or a WebP, lists every metadata
block in it with where it is and whether it is AI provenance — and why —
and writes the file back without the blocks a caller chose to drop,
every other byte unchanged.

## §2 Read first

- `CLAUDE.md`: "The third shelf is never empty", "Only applications
  localize", "The bytes decide what a thing is", "Tests must be able to
  fail", the dependency rules.
- `crates/wipemark-image/src/lib.rs` — the E0 vocabulary.
- `docs/plan/README.md` §7 "E11"; the overview's §9 (in the task).
- `crates/wipemark-intake/src/{format.rs,magic.rs}` — how an image is
  recognised today.
- The container specifications: PNG (W3C PNG 3rd ed.: chunk layout,
  critical vs ancillary, `eXIf`, `iTXt`), JPEG (ITU T.81 B.1: markers,
  fill bytes, byte stuffing, RSTn), JPEG XT / ISO 19566-5 (JUMBF in
  APP11), RIFF/WebP container (Google's spec: `VP8X` flags), C2PA 2.x
  §"Embedding manifests" (PNG `caBX`, JPEG APP11, WebP `C2PA`), IPTC
  Photo Metadata `DigitalSourceType` and its NewsCodes vocabulary.

## §3 What is true today

- `crates/wipemark-image/src/lib.rs:20-33` — `ImageContainer` (six
  variants); `:36-57` — `MetadataKind` with `is_ai_provenance` true for
  `C2pa` and `GeneratorParameters` only; `:60-67` — `MetadataFinding
  { kind, chunk, offset, len }`; `:70-79` — `StripReport { container,
  removed, kept, still_has_c2pa, still_has_ai_metadata }`; `:81-92` —
  `ImageError { UnknownContainer, Malformed { container, offset, detail:
  String }, NotImplemented(&str) }`. No parser, no caller.
- `scripts/check-dep-direction.sh:100` — `"wipemark-image":
  {"wipemark-core"}`; `:108` — "`wipemark-image` is what opens a
  container, and knowing one is there is not opening it."
- `crates/wipemark-intake/src/magic.rs:51,66,171` — the PNG, JPEG and
  WebP signatures as the recogniser knows them.
- `crates/wipemark-core/src/report.rs:19-45` — `not_established::ALL`,
  three ids, each with a translation in every catalogue (gated by
  `wipemark-i18n`).
- `fixtures/README.md:19` — `image/` announced for "PNG/JPEG/WebP/TIFF
  carrying C2PA manifests, XMP DigitalSourceType, Stable Diffusion
  `parameters` blocks", empty.

## §4 Decisions

Numbered `I1…` here; the coordinator assigns D-numbers in
`docs/plan/README.md` §4.

| # | decision | why |
|---|---|---|
| I1 | **Every byte of the input belongs to exactly one block**, and a block is either *structure* (never a finding, never removed) or *metadata* (a finding). `strip` is "keep these blocks, concatenate", plus the two WebP fix-ups (I9). `blocks_tile_the_file` checks the tiling for every fixture. | It is what "every other byte unchanged" means as an invariant rather than a hope: a byte that belongs to no block cannot be copied by accident, and one that belongs to two cannot be dropped by accident. |
| I2 | Structure is: PNG — signature, `IHDR`, `PLTE`, `IDAT`, `IEND`, `tRNS`, the APNG chunks, and **any critical chunk** (uppercase first letter) this build does not know; JPEG — SOI, every marker that is not APPn/COM, each SOS with its entropy-coded data, EOI, **APP0** (JFIF/JFXX), **APP14 `Adobe`** (the colour transform a decoder needs) and **APP2 `MPF`** (I8); WebP — the RIFF header, `VP8X`, `VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`. | A critical PNG chunk is by definition one a decoder cannot skip; APP14 decides how YCC/CMYK is decoded, so dropping it changes the decoded raster — the one thing this crate promises not to do. |
| I3 | A new kind, **`MetadataKind::Rendering`**: `iCCP`, `gAMA`, `cHRM`, `sRGB`, `cICP`, `mDCV`, `cLLI`, `sBIT`, `bKGD`, JPEG APP2 `ICC_PROFILE`, WebP `ICCP`. Listed by `inspect`, **never removed, not even by `Scope::AllMetadata`**. A second new kind, **`Other`**: metadata that is not text (`pHYs`, `tIME`, unknown ancillary chunks, unknown APPn, WebP `PSAI`, a trailer). | Removing a colour profile changes how the picture *looks* while leaving the raster's hash alone — a gate that cannot see the change is exactly where a promise has to be stated instead. "Strip every non-essential chunk" therefore keeps what rendering needs, and says so. |
| I4 | **One knob, not two booleans**: `StripOptions { scope: Scope }`, `Scope::AiProvenance` (default) or `Scope::AllMetadata`. | The overview's `keep_non_ai_metadata` and `strip_all_metadata` are one choice spelled twice; two booleans have a fourth state (`keep` *and* `strip all`) that means nothing. |
| I5 | **A finding is AI provenance by its kind or by its evidence**: `MetadataFinding::is_ai_provenance()` is `kind.is_ai_provenance() || !evidence.is_empty()`. `Evidence { signal, field, matched }` names the signal (`C2paManifest`, `C2paReference`, `DigitalSourceType(SourceType)`, `GeneratorKey(Generator)`, `GeneratorText(Generator)`), the field it was found in (a PNG keyword, `EXIF`, `XMP`, `IPTC`, `C2PA`, `COM`) and **the signature that matched — never the value** (a prompt is the user's text and does not belong in a report). | An XMP packet is camera data until it carries `trainedAlgorithmicMedia`; the kind says what a block is, the evidence says why it goes. |
| I6 | **Signatures are data** (`signatures.rs`): `KEYWORDS` (a PNG text key that is itself a generator's: `parameters`, `prompt`, `workflow`, `invokeai_metadata`, `invokeai_graph`, `sd-metadata`, `Dream`), `TEXT` (needles, all of which must occur, each limited to the places it may be believed in), `SOURCE_TYPES` (the three AI codes of IPTC's vocabulary). Every entry carries its evidence as a sentence for the next reader. A broad name (`ChatGPT`, `OpenAI`) is believed **only inside a C2PA manifest**; a product name (`Adobe Firefly`, `Midjourney`, `DALL·E`, `NovelAI`, …) anywhere. Matching is case-sensitive and over the UTF-8, UTF-16LE and UTF-16BE spellings of a needle, so an EXIF `UserComment` written as `UNICODE` is read without parsing its IFD. | The list will grow and will be argued about entry by entry; a table with its reasons is the shape that survives that. A needle believed everywhere has to be specific enough that a human caption is unlikely to carry it. |
| I7 | **A `DigitalSourceType` matches only as an IPTC URI**: `digitalsourcetype/<code>` followed by a byte that cannot continue a name. **`dcterms:provenance`** in an XMP packet is `C2paReference` and counts as C2PA for `has_c2pa`. | `algorithmicMedia` is a substring of nothing else in the vocabulary only by luck of capitalisation; the URI form is what IPTC specifies. A provenance pointer to a manifest — remote, or `self#jumbf=` to one already removed — binds the file as surely as the manifest did (the C2PA sample `xmp-provenance.jpg` is exactly that). |
| I8 | **JPEG**: all XMP segments (standard and extension) are one packet — the evidence is computed over their concatenation and given to every one of them, so they leave together. APP11 JPEG XT segments sharing a box instance number (`En`) with a segment labelled `c2pa` are C2PA. **If an `MPF` segment is present, removing a segment after it is refused** (`ImageError::Unsupported(MultiPicture)`) — MPF offsets are relative to its own header, so a removal between that header and the secondary images moves every picture after the first. Bytes after EOI are a *trailer* (`Other`), kept by `AiProvenance`, removed by `AllMetadata` unless an `MPF` segment says they are pictures. | Editing inside XMP is out of scope, so XMP is kept or removed whole — and half an extended packet is a packet nobody can read. A refusal is honest where a rewrite of MPF offsets would be a second container writer. |
| I9 | **WebP**: the RIFF size is rewritten **only when a chunk was removed**, as 4 + the kept chunks; the `VP8X` flags byte is changed **only** to clear the EXIF (`0x08`) or XMP (`0x04`) bit when no such chunk remains, never to set one. A RIFF size past the end of the file is `Malformed(RiffSize)`; bytes after the RIFF are a trailer as in I8. | The only two bytes this crate may compute, and both are stated. |
| I10 | **PNG**: chunks and their CRCs are copied untouched and CRCs are **not** verified — a damaged CRC is the decoder's business, and refusing would make a cleaner fail on files every viewer opens. `IHDR` must be first and `IEND` must exist; bytes after `IEND` are a trailer as in I8. Text: `tEXt` (Latin-1), `zTXt` and compressed `iTXt` inflated with a limit (`INFLATE_LIMIT`, 16 MiB) through `miniz_oxide`; a compressed text that does not inflate, or inflates past the limit, is `Malformed` — **a chunk that could not be read is not a chunk found clean**. ImageMagick's `Raw profile type <x>` hex payloads are decoded before they are searched. | The keyword alone is enough for `parameters`, but not for a `Software` value or an XMP packet inside a `zTXt`. |
| I11 | **Errors are values**: `ImageError::Malformed { container, offset, defect: Defect }` with an exhaustive `Defect` (truncated, bad signature, IHDR not first, no end, bad length, bad marker, RIFF size, bad text, inflate, inflate limit); `ImageError::Unsupported { container, offset, what }`; `ImageError::NotYet(ImageContainer)` for TIFF/HEIC/AVIF, which `ImageContainer::sniff` recognises so the refusal can name them. E0's `detail: String` and `NotImplemented(&str)` are gone. | Only applications localize; a surface will want to say "this JPEG ends before its last picture" in three languages, and a `String` is a sentence the library already chose. |
| I12 | **`still_has_*` and `kept` come from a second `inspect` of the output** — offsets in `kept` are offsets in the output. `removed` carries offsets in the input. If nothing is selected for removal the output **is** the input, byte for byte. | The task's rule; it is also what makes `strip(strip(x)) == strip(x)` a test rather than a claim. |
| I13 | **The third shelf** is `wipemark_core::report::not_established::ALL`'s ids, all three, on every report: `unknown-mark-schemes` is the pixel domain (`PIXEL_DOMAIN`), `vendor-detector-evasion` and `human-authorship` apply to a picture as they do to text. No new id, so no catalogue edit. | Reuse was preferred by the task; the existing sentence ("marks in schemes this build does not implement — not searched for") is exactly the claim. |
| I14 | Fixtures live in **`fixtures/image/`** (singular), not `fixtures/images/`. | `fixtures/README.md` already announces `image/`; the README wins over the task's spelling. |

## §5 Deliverables

1. `inspect(bytes) -> Result<ImageReport, ImageError>` — `ImageReport {
   container, findings, not_established }`, `has_c2pa()`,
   `has_ai_metadata()`. Never modifies anything.
2. `strip(bytes, &StripOptions) -> Result<(Vec<u8>, StripReport),
   ImageError>` per I1, I8, I9, I12.
3. `signatures` — the tables of I6/I7 as `pub const` data.
4. `ImageContainer::sniff` — PNG, JPEG, WebP, and the three formats that
   are refused by name (I11).
5. Fixtures: four real files from the C2PA project (`fixtures/image/`,
   README with origin, commit and licence) and builders in
   `tests/support/` for everything injected.
6. `docs/architecture/images.md` (new) and the report
   `docs/plan/reports/E11-1-2026-10-04.md`.

## §6 Tests (RED first; a mutation per protection)

| test | protection | mutation that must turn it red |
|---|---|---|
| `pixels_never_change`, `image_data_is_byte_identical` (tests/pixels.rs) | the raster's sha256 and the image-data bytes are identical after `strip` | flip a byte of every large kept structure block in `concat` |
| `every_removed_block_is_exactly_the_bytes_that_went` | the output is the input minus the reported ranges, plus the five WebP bytes | same |
| `blocks_tile_the_file` | I1 | leave a WebP pad byte out of its block |
| `each_ai_signal_is_found`, `each_ai_signal_is_removed_by_default_and_read_off_the_output` | I5–I7 for C2PA ×3 containers, XMP DST, IPTC DST, `parameters`, ComfyUI `prompt`/`workflow`, a generator `Software` | empty `KEYWORDS`; drop the UTF-16 spellings; read `caBX` as `Other` |
| `a_source_type_matches_only_as_the_uri_and_only_whole` | I7 | drop the boundary check |
| `non_ai_metadata_is_kept_by_default_and_removed_by_all`, `colour_is_never_removed` | I3, I4 | make `Rendering` removable; make `AiProvenance` remove `Exif` |
| `still_has_is_read_off_the_output` (unit) | I12 | return `false` instead of the second inspection |
| `strip_is_idempotent`, `something_to_strip_makes_the_file_shorter_and_nothing_to_strip_keeps_it_whole` | I12 | make `AiProvenance` drop `Other` |
| `vp8x_flags_follow_what_remains`, `a_flag_that_was_already_wrong_is_not_this_passes_to_fix` | I9 | skip the flag patch; clear a bit for a kind not removed |
| `jpeg_xmp_leaves_whole` | I8 | evidence per segment instead of per packet |
| `mpf_offsets_are_never_moved` | I8 | remove the MPF refusal |
| `every_truncation_is_refused`, `no_mutation_panics`, `a_webp_whose_riff_size_lies_is_refused`, `a_jpeg_without_eoi_is_refused` | I11, malformed input | accept a JPEG that ends between segments; clamp the RIFF size (a panic is red by itself) |
| `every_report_keeps_the_pixel_domain_on_the_third_shelf` | I13 | return an empty shelf |
| `a_compressed_text_that_cannot_be_read_is_refused` | I10 | read a failed inflate as empty |

## §7 Acceptance

- `inspect` lists every metadata block of every fixture with kind,
  chunk, offset, length and evidence.
- `strip` with defaults removes every AI signal of §6 and keeps camera
  EXIF, ICC, gamma/sRGB, `pHYs`; `AllMetadata` keeps only structure and
  `Rendering`.
- The decoded raster's sha256 is identical for every fixture and every
  scope.
- Truncating any fixture at any offset is an `Err`; flipping any byte
  never panics.
- All gates of §0.4 green.

## §8 Out of scope

TIFF, HEIC/AVIF (E11-2 — refused by name today), any surface, the pixel
domain, re-encoding of any kind, editing inside an XMP packet or an EXIF
IFD (a block is kept or removed whole), CRC repair, MPF offset rewriting.

## §9 Basis

The task (Watchword `wipemark-task-e11-1-image-metadata-2026-10-04`);
OV §9, §11; W3C PNG 3rd edition; ITU-T T.81; ISO/IEC 19566-5 (JUMBF);
Google's WebP container specification; C2PA Technical Specification
2.1 §"Embedding manifests into assets"; IPTC Photo Metadata Standard
2024.1 and the `digitalsourcetype` NewsCodes; CIPA DC-008 (EXIF
`UserComment` character codes); Adobe XMP Specification Part 3 (JPEG
extended XMP).
