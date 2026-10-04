# Images — provenance metadata in PNG, JPEG and WebP

`crates/wipemark-image`: what an image file says about where it came
from, and the same file without it. Built in E11-1
(`docs/plan/E11-1-image-metadata.md`, report
`docs/plan/reports/E11-1-2026-10-04.md`). Two surfaces call it (E11-2,
`docs/plan/E11-2-images-on-the-surfaces.md`): `wipemark-cli inspect|clean|audit`
and the MCP tools `inspect_image` and `clean_image` — see "Surfaces" below.
No window does yet: not the queue, not the panel (E7).

## The promise, and how it is kept

**Pixels are never re-encoded.** The crate never decodes a pixel: it
cuts a file into blocks, drops some, and concatenates the rest. The gate
(OV §11) decodes the raster before and after with three independent
decoders — `png`, `zune-jpeg`, `image-webp`, dev-dependencies only — and
compares sha256s, for every fixture under both scopes
(`tests/pixels.rs`, `pixels_never_change`); and it compares the
image-data bytes themselves — IDAT, every scan with its entropy-coded
data, VP8/VP8L/ALPH/ANMF — with walkers in `tests/support/` that share
no code with the parsers (`image_data_is_byte_identical`). A walker that
reused the parser would agree with it about exactly the bugs it is there
to catch.

The mechanism is one invariant: **the blocks tile the file**. Every
byte belongs to exactly one block (`lib.rs`, `tiles`, asserted in debug
builds on every parse and by `blocks_tile_the_file`). A block is
*structure* — never listed, never removed — or *metadata*, which is a
`MetadataFinding`. `strip` keeps the blocks it was not asked to drop and
concatenates them byte for byte (`concat`). The only bytes this crate
ever computes are a WebP's RIFF size and two bits of its `VP8X` flags,
and only when a chunk was removed, and a JPEG's MP Index size of the
first picture, and only when a block before its `MPF` header was removed
(E11-3, below); `every_removed_block_is_exactly_the_bytes_that_went`
checks that the output is the input with the reported ranges cut out and
nothing else changed but those five bytes, and
`the_mp_index_follows_a_removal_before_it` the same for the four.

When nothing is selected the output **is** the input: no rebuild at all.

## What is structure, what is metadata

| | structure (never touched) | `Rendering` (listed, never removed) | metadata |
|---|---|---|---|
| PNG | signature, `IHDR`, `PLTE`, `IDAT`, `IEND`, `tRNS`, `acTL`/`fcTL`/`fdAT`, **any critical chunk** (uppercase first letter) | `iCCP`, `gAMA`, `cHRM`, `sRGB`, `cICP`, `mDCV`, `cLLI`, `sBIT`, `bKGD` | `caBX` (C2PA), `eXIf`, `tEXt`/`zTXt`/`iTXt`, `pHYs`, `tIME`, any other ancillary chunk, bytes after `IEND` |
| JPEG | SOI, EOI, every non-APP marker (DQT, SOFn, DHT, DRI, …), each SOS **with its entropy-coded data**, RSTn, APP0 (JFIF), APP14 `Adobe`, APP2 `MPF` and, when MPF is present, everything after EOI | APP2 `ICC_PROFILE` | APP1 `Exif`, APP1 XMP and extended XMP, APP11 JPEG XT (C2PA when labelled), APP13 `Photoshop 3.0` (IPTC), COM, other APPn, bytes after EOI without MPF |
| WebP | the RIFF header, `VP8X`, `VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF` | `ICCP` | `EXIF`, `XMP `, `C2PA`, any other chunk (Photoshop's `PSAI`), bytes after the RIFF |

Why these lines:

* **APP14 `Adobe` is structure** because it tells a decoder whether the
  samples are YCC or RGB/CMYK; dropping it changes the decoded raster.
* **`Rendering` is never removed, even by `AllMetadata`.** A colour
  profile changes how the picture *looks* and not one decoded sample, so
  the gate cannot see its loss — which is exactly where a promise has to
  be stated instead of tested. "Every non-essential chunk" therefore
  means "every chunk that is neither structure nor rendering".
* **An unknown critical PNG chunk is structure**: by the PNG
  specification, a decoder that does not know it must refuse the file,
  so it cannot be decoration (`an_unknown_critical_chunk_is_structure`
  feeds `CgBI` and `ABCD` after `IHDR`). Apple's real `CgBI` comes
  *before* `IHDR`, and such a file is refused as `HeaderNotFirst` — it is
  not a PNG a standard decoder reads either.
* **MPF** (CIPA DC-007, a JPEG with secondary pictures after EOI) holds
  offsets relative to its own header. Removing a segment *before* that
  header moves everything together and is allowed; removing one *after*
  it would move the pictures and is refused
  (`ImageError::Unsupported { what: MultiPicture }`) — rewriting MPF
  offsets would be a second container writer, and E11-1 has one. One
  field does change with a removal before the header: the MP Index's
  **Individual Image Size** of the first picture, which counts SOI to EOI.
  It is rewritten (E11-3) to the old value minus the bytes removed, in
  the index's own byte order; an index whose field cannot be read — its
  IFD or its entry outside the segment, a byte order neither `II` nor
  `MM` — or whose size is smaller than what went refuses the removal as
  `MultiPicture` too, rather than leaving a field nobody checked. An
  index with no MP Entry has nothing to go stale. Only the **first**
  `MPF` header counts: the index lives in the first picture's.

## What makes a block AI provenance

A finding is AI provenance by its **kind** (`C2pa`,
`GeneratorParameters`) or by its **evidence**
(`MetadataFinding::is_ai_provenance`). The evidence says which signal,
in which field, and *which signature* matched — never the value: a
prompt is the user's own text and does not belong in a report
(`the_report_never_quotes_the_value`).

| signal | where it is looked for | how |
|---|---|---|
| `C2paManifest` | PNG `caBX`, JPEG APP11 JPEG XT (`JP` + `jumb`, labelled `c2pa` within the first 96 bytes, or sharing a box instance number `En` with a segment that is), WebP `C2PA` | the container's own chunk name; then generator names inside the manifest as further evidence |
| `C2paReference` | XMP only | `dcterms:provenance` — a pointer to a manifest, remote or `self#jumbf=…` to one already removed, binds the file as surely as the manifest did. `has_c2pa` counts it |
| `DigitalSourceType` | every block | `digitalsourcetype/` + `trainedAlgorithmicMedia`, `compositeWithTrainedAlgorithmicMedia` or `algorithmicMedia`, followed by a byte that cannot continue a name |
| `GeneratorKey` | PNG text keys | `parameters` (stable-diffusion-webui), `prompt` and `workflow` (ComfyUI), `invokeai_metadata`, `invokeai_graph`, `sd-metadata`, `Dream` (InvokeAI) |
| `GeneratorText` | per entry | needles, all of which must occur, each believed only in the places its entry lists — the A1111 infotext line, `NovelAI`, `Midjourney`, `DALL·E`/`DALL-E`, `Adobe Firefly`, `Made with Google AI`, `Edited with Google AI`, `Microsoft Designer`, `Bing Image Creator`; `OpenAI` and `ChatGPT` **only inside a C2PA manifest**, because the bare words are ordinary prose anywhere else |

The tables are data, in `src/signatures.rs`, every entry with a sentence
saying why it is believed. Matching is case-sensitive and over UTF-8,
UTF-16LE and UTF-16BE — so an EXIF `UserComment` that
stable-diffusion-webui wrote as `UNICODE` is read without parsing an IFD.
That is deliberate: EXIF and IPTC are searched as bytes, not parsed. A
signature anywhere in the block counts. The one EXIF field read as a
field is IFD0's **Orientation** (`exif.rs`, bounded, never a panic), and
only so a strip can say the rotation went with a block — see "Scopes".

**To add a signature**: one entry in `KEYWORDS` or `TEXT`, with its
evidence; a case in `tests/support::injected` that carries it; a line in
`each_ai_signal_is_found`. Prefer a product name to a company name, and
limit a word that could be prose to `Place::C2pa`.

Two choices in how a block is read:

* **JPEG XMP leaves whole.** The main packet and every extended-XMP
  segment are one packet: the evidence is computed over their
  concatenation and given to each (`jpeg.rs`, `settle`), so they are kept
  or removed together. Editing *inside* an XMP packet is out of scope — a
  packet is kept or removed whole, which can take a camera's XMP fields
  with an AI Digital Source Type. That is the price of not writing XML.
* **Compressed text is read, or the file is refused.** `zTXt` and
  compressed `iTXt` are inflated through `miniz_oxide` up to
  `INFLATE_LIMIT` (16 MiB); a stream that does not inflate, or inflates
  past it, is `Defect::Inflate`/`InflateLimit`. A chunk that could not be
  read is not one found clean. ImageMagick's `Raw profile type <x>` hex
  is decoded before it is searched.

## Scopes

`StripOptions { scope }` — one knob, not the overview's two booleans
(`keep_non_ai_metadata` and `strip_all_metadata` are one choice spelled
twice, and the fourth combination meant nothing):

* `Scope::AiProvenance` (default) removes every AI-provenance finding
  and nothing else — camera EXIF, orientation, ICC, gamma, `pHYs`, an
  unrelated `Comment` stay.
* `Scope::AllMetadata` removes every finding but `Rendering`. It takes
  EXIF whole, orientation with it.

Under either scope an EXIF block leaves whole — the default takes one
when it names a generator. When a removed EXIF block carried an
Orientation other than 1 (2 to 8: turned or mirrored), the strip says
so: `StripReport::orientation_removed` is that value, from the first such
block in file order. It is a fact about what went, not a promise about
what a viewer does; nothing is edited *inside* a block to keep it.

`still_has_c2pa`, `still_has_ai_metadata` and `kept` come from a
**second `inspect` of the output** (`finish`), never from what was
removed; `kept`'s offsets are the output's, `removed`'s the input's.

## Honesty

Every `ImageReport` and `StripReport` carries `not_established`: all
three ids of `wipemark_core::report::not_established::ALL`, and
`PIXEL_DOMAIN` (`unknown-mark-schemes`) among them, because a picture
whose metadata is clean can still carry a mark in its pixels and this
crate does not look. "No AI metadata found" is about metadata; nothing
here says anything about the picture. No new id was added, so no
catalogue changed.

## Malformed input

Refused as a value — `ImageError::Malformed { container, offset, defect }`
with an exhaustive `Defect` — never a panic and never a partial output.
`tests/malformed.rs` cuts every small case at every offset (each cut is
an `Err`: the end marker is gone) and flips, zeroes and maxes every byte
under both scopes (no panic). PNG CRCs are **not** verified: a damaged
CRC is the decoder's business and the bytes are never changed. A WebP
whose RIFF size claims more than the file holds is `RiffSize`; bytes
*after* a truthful RIFF are a trailer, like bytes after `IEND` or EOI.

TIFF, HEIC and AVIF are recognised by `ImageContainer::sniff` and refused
as `ImageError::NotYet(container)` — the surface can say "not in this
version yet" by name. Recognising what a dropped thing *is* stays with
`wipemark-intake`; `sniff` is only the signature a parser needs anyway.

## Dependencies

`wipemark-core` (the third shelf), `thiserror`, and `miniz_oxide` for
inflate — pure Rust, already in the tree through gpui's `png`.
Dev-only: `png`, `zune-jpeg`, `image-webp`, `sha2`, all already in the
tree. `scripts/check-dep-direction.sh` keeps `wipemark-image` →
`wipemark-core` the only workspace edge.

## Fixtures

`fixtures/image/`: four real files from `contentauth/c2pa-rs`'s public
test fixtures (MIT / Apache-2.0), with origin, commit, hashes and what
each asserts in `fixtures/image/README.md`. Everything else is built by
`tests/support/` around a picture encoded at test time — a 5×4 PNG, a
6×3 lossless WebP, a hand-written 8×8 baseline JPEG, and the real JPEG
with its APP segments taken out — so injected cases are code, not blobs.

## JSON

`ImageReport::to_json` and `StripReport::to_json` (`src/json.rs`) — one
writer for every surface, `std` alone, as core's is for a text (D9). One
line, keys in a fixed order, ASCII; the third shelf written from core's
constant as the last key, so no surface can drop it; and every string
that came out of the file — `chunk`, `key`, an evidence's `field` — spelled
by `spell`: printable ASCII as itself, anything else `U+XXXX`. A PNG
keyword is Latin-1 and can carry a soft hyphen or a C1 control; a report
handed to a model must not carry the marks it reports.

```json
{"container":"png","ai_metadata":true,"c2pa":false,
 "findings":[{"kind":"generator-parameters","chunk":"tEXt","key":"parameters",
   "offset":33,"length":40,"ai":true,"c2pa":false,
   "evidence":[{"signal":"generator-key","generator":"stable-diffusion-webui",
     "source_type":null,"field":"parameters","matched":"parameters"}]}],
 "not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

`StripReport`: `container`, `still_has_ai_metadata`, `still_has_c2pa`,
`removed` (input offsets), `kept` (output offsets), `orientation_removed`
(the EXIF value 2–8, or `null`), `not_established`. The
ids — `ImageContainer::id`, `MetadataKind::id`, `Signal::id`,
`Defect::id`, `Scope::id`, `Generator::id`, `SourceType::code` — are
formats, never translated; a surface keys its words off them
(`image-kind-<id>`, `image-signal-<id>`, `image-defect-<id>`).

## `reframe` — the one writer for changed pixels (E12-3)

`reframe(original, new_image, &StripOptions)` (`src/reframe.rs`): the new
file's **structure** inside the original's **rendering and metadata** —
the latter filtered by the scope exactly as `strip` filters it, in the
original's order, byte for byte. It is what `wipemark-picture` writes a
restored picture with (D159), and it still decodes nothing. The gate:
framing a file in itself is stripping it — `reframe(x, x) == strip(x)`,
bytes and report, for every fixture under both scopes.

* **PNG**: `IHDR`, `PLTE`, `tRNS`, the `IDAT`s and `IEND` come from the
  new file, where the original had them (a `PLTE`/`tRNS` the original
  lacked goes before the image data). A colour-type or bit-depth change
  takes `bKGD`, `sBIT` and `hIST` with it — their bytes are in the old
  type's terms — and lists them as removed: the one place a `Rendering`
  block leaves, and only because it would be wrong.
* **WebP**: the image chunks (`VP8`/`VP8L`/`ALPH`) come from the new file;
  `VP8X` stays, its EXIF/XMP bits cleared as `strip` clears them and its
  alpha bit set when the new image has alpha; the RIFF size counts what is
  there.
* **JPEG**: the coding segments (APP0 JFIF, APP14 Adobe, tables, frame,
  scans, EOI) come from the new file; metadata that stood among the
  original's coding segments moves ahead of them.
* Refused as `Unsupported::Reframe`: animation (`acTL`/`fcTL`/`fdAT`,
  `ANIM`/`ANMF`), an unknown critical chunk, a new file of another
  container. An MPF JPEG stays `MultiPicture`. The surfaces word both.

## Surfaces

**The command line** (`apps/wipemark-cli/src/image.rs`; the whole table is
`docs/architecture/cli.md`, "Images"). A file is a picture when intake
places it as one **by its bytes**. `inspect` exits 1 when any block is AI
provenance and 0 otherwise — camera EXIF is not a finding. `clean` writes
`name.cleaned.ext` beside the input (or `-o`, stdout when it is not a
terminal, `--in-place` through `wipemark_intake::inplace`) and exits by the
input — 1 when it carried AI provenance — except that an output for which
`still_has_*` is true is **not written** and exits 3. `--all-metadata` is
`Scope::AllMetadata`; its help says it takes EXIF orientation, and that
colour is kept; the report says "the picture's rotation was in the
removed camera data" when `orientation_removed` is set, in either scope,
and never because an EXIF block merely went. TIFF/HEIC/AVIF exit 2 "not in this
version yet"; a malformed picture exits 3 with its defect. `audit` inspects
the pictures in a folder: a finding is AI provenance, a malformed picture
is a hole (3 beats 1), a TIFF is skipped (`image-not-yet`), and SARIF puts
a block at `region.byteOffset`/`byteLength`.

**MCP** (`apps/wipemark-app/src/mcp/image.rs`). `inspect_image { data }`
answers `ImageReport::to_json()`; `clean_image { data, scope }` answers
`{"data": <base64>, "report": <StripReport>}` — both as `content[0].text`
and `structuredContent`, ASCII to the byte. `data` is base64, standard
alphabet, padded, strictly (`base64` 0.22, already in the lock); `scope`
is `ai-provenance` (default) or `all-metadata`. The transport's 1 MiB is
the limit — about 750 KB of picture — and a larger body is a `413`, never
a truncation. Every way a call cannot run is an `isError` result naming
why: not base64, not a picture this server reads (naming what intake found),
TIFF/HEIC/AVIF, a malformed picture, an MPF index a removal would leave wrong,
and a result that would still carry AI provenance — for which no image
comes back. **There is no `path` argument**: a server that can be bound
past loopback with no password must not read or write files by name, and
whether it ever should is an owner question.

Both surfaces say, in every report, that only the metadata was examined:
a mark in the pixels is not looked for, and the third shelf names it.

## Next

TIFF (IFDs — a strip there *is* an IFD rewrite, unlike here), then
HEIC/AVIF (ISOBMFF `meta`/`uuid`, `iloc` offsets that move when a box
goes) — both in the backlog, built only on demand (owner: "AI generates
JPEG and PNG"). The windows: the queue and the panel showing a picture's
report (E7).
