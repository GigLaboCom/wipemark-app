# E12-4 — JPEG and lossy WebP: decode, restore, re-encode

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/E12-visible-marks.md` — epic E12; step 5 of the images series                                                  |
| Spec scopes      | `docs/sdd/visible-marks.md` §5.1, §5.2 (superseded by the owner's answer below), §5.3; D158 and D159 as amended          |
| Task             | Watchword FILE `wipemark-task-images-series-2026-10-04`, step 5                                                           |
| Depends on       | E12-3 (`wipemark-picture`, `wipemark_image::reframe`)                                                                    |
| Files touched    | `crates/wipemark-picture/{Cargo.toml,src/{lib,decode,encode}.rs,tests/lossy.rs}`, `Cargo.lock`, `docs/architecture/visible-marks.md`, this document, its report |
| Size             | ~1 day (the owner made it small)                                                                                          |

## §0 Ground rules

`CLAUDE.md` wins. One commit, `E12-4: JPEG and lossy WebP — decode,
restore, re-encode`. Tests compiled, not run.

**The owner's answers (2026-10-04), binding:** "removal with no
parameters: marks found are removed; re-encoding and the like do not
matter." Q-V2: a full re-encode is fine — decode, restore, re-encode at
high quality with `image`'s `JpegEncoder`, quality about 95, the input's
chroma subsampling where the encoder allows; **no coefficient codec, no
block patch** (D158 amended). Q-V3: a lossy WebP comes out as lossless
WebP.

## §1 Goal

A JPEG or lossy WebP carrying a verified mark comes out restored: JPEG
re-encoded at quality 95, lossy WebP written as lossless `VP8L`, the
metadata the scope keeps carried over by `reframe` (EXIF, ICC, XMP), C2PA
dropped (D159), and the report saying the picture was re-encoded.

## §3 True today

* `wipemark_picture::clean` (`crates/wipemark-picture/src/lib.rs`) examines
  a JPEG or lossy WebP and reports its marks with `restorable: false`.
* `wipemark_image::reframe` already frames a JPEG around new scans and a
  WebP around new image chunks.

## §4 Deliverables

1. `decode` remembers a JPEG's component count; a **grey** JPEG is
   re-encoded grey (`L8`), a YCbCr one as RGB; a **CMYK** JPEG is examined
   and not restored (its colour profile, which `reframe` keeps, describes
   inks the re-encoded file would not have).
2. `encode_like` for JPEG: `image::codecs::jpeg::JpegEncoder`, quality
   `JPEG_QUALITY = 95`. The encoder offers no subsampling choice; it
   writes 4:4:4, which loses less than any input's subsampling.
3. Lossy WebP: `restorable`; written lossless (`Encoding::WebPLossless {
   from_lossy: true }`).
4. The proof for a lossy output: the output decodes to within
   `PSNR_FLOOR = 34 dB` of the restored raster (it cannot be identical);
   nothing outside the rectangles moved in the restored raster; nothing
   verifies on the output.

## §5 Tests

| test | protects | mutation |
|---|---|---|
| `a_marked_jpeg_is_restored_and_re_encoded` | the mark is gone by the verifier's own test; outside the mark the output is within the encoder's tolerance (PSNR printed, ≥ 34 dB asserted); EXIF and ICC byte-identical; the AI XMP gone; `Encoding::Jpeg { quality: 95 }` | JPEG not restorable |
| `a_grey_jpeg_stays_grey` | one component in, one out | always RGB |
| `a_lossy_webp_is_written_lossless` | VP8 and ALPH replaced by VP8L, EXIF kept, the decode equal, `from_lossy` | `from_lossy: false` |
| `the_lossy_proof_refuses_a_distant_output` | the PSNR floor | drop the check |

## §7 Out of scope

A coefficient codec or block patch (the owner's answer); progressive
output; surfaces (E12-5).
