# E12-3 — `wipemark-picture`: a picture file through the pixels pass

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/E12-visible-marks.md` — epic E12; step 4 of the images series                                                  |
| Spec scopes      | `docs/sdd/visible-marks.md` §4.1, §4.3 (steps 0 and 6–8), §5.1, §5.3, §9 "Containers"; D150, D157–D159                    |
| Task             | Watchword FILE `wipemark-task-images-series-2026-10-04`, step 4                                                           |
| Depends on       | E12-1 (`wipemark-pixels`), E11-1/E11-3 (`wipemark-image`)                                                                 |
| Unblocks         | E12-4 (JPEG and lossy WebP), E12-5 (the surfaces)                                                                        |
| Files touched    | `crates/wipemark-image/src/{lib,reframe}.rs`, `crates/wipemark-image/tests/reframe.rs`, `crates/wipemark-picture/**` (new), `Cargo.toml`, `Cargo.lock`, `scripts/check-dep-direction.sh`, `apps/wipemark-cli/src/image.rs`, `apps/wipemark-app/src/mcp/image.rs`, the three catalogues (one refusal), `docs/architecture/{images,visible-marks}.md`, this document, its report |
| Size             | ~3 days                                                                                                                  |

## §0 Ground rules

`CLAUDE.md` wins. One commit, `E12-3: Picture files — PNG and lossless WebP
through the pixels pass`. `wipemark-image` still never decodes a pixel; it
gains one writer, `reframe`. `wipemark-picture` is the only crate that
decodes and encodes a picture, and the only one with all three of
`image`, `pixels` and `core` below it. Tests compiled, not run.

## §1 Goal

`wipemark_picture::clean(bytes, &PictureOptions) -> (bytes, PictureReport)`:
the metadata pass of E11 and the visible pass of E12-1 in one call with
one writer. For PNG and lossless WebP a verified mark is restored and
the file is written back losslessly; for JPEG and lossy WebP the visible
pass examines and reports (restoring is E12-4).

## §3 True today

* `wipemark_image::strip` (`crates/wipemark-image/src/lib.rs`) is the only
  writer; it concatenates kept blocks.
* `wipemark_pixels::{examine, clean}` work on a `Raster`; nothing decodes
  a file into one.

## §4 Deliverables

1. **`wipemark_image::reframe(original, new_image, &StripOptions)`** — the
   new file's structure in the original's metadata and rendering (see
   `crates/wipemark-image/src/reframe.rs` for each container);
   `Unsupported::Reframe` for animation, an unknown critical chunk, or a
   different container; MPF stays `MultiPicture`. A colour-type change
   takes `bKGD`, `sBIT` and `hIST` (their bytes are in the old type's
   terms) and lists them as removed.
2. **`wipemark-picture`**: `decode` (the stored raster: palette and grey
   expanded for the maths, 16 bits kept, no colour management, no
   rotation, alpha kept), `encode_like` (PNG at the original's colour type
   and depth where the new values allow — a palette stays a palette when
   every restored colour is in it, grey stays grey when every pixel is
   grey; otherwise RGB(A), said; interlace is not written, said; lossless
   WebP), `inspect` and `clean`.
3. **One pass, one writer (D159).** Metadata is inspected on the
   original; pixels are restored from the original; the result is written
   once by `reframe`. Nothing restored → the output is `strip`'s, byte for
   byte. C2PA leaves whenever pixels changed (both scopes remove it).
4. **Prove before writing** (SDD §4.3 step 8): decode the output and
   require it to equal the restored raster sample for sample; require
   every sample outside the restored rectangles to equal the input's;
   re-examine the output and require that nothing verifies. A failure is
   `PictureError::Proof`, no output.
5. **What was not examined is said**: an animated picture, a catalogue
   that did not load — `Visible::NotExamined(why)`, and the metadata pass
   still runs.
6. `PictureReport::to_json()` — `container`, `metadata` (E11's
   `StripReport` JSON), `visible` (E12-1's `PixelReport` JSON or the
   reason it did not run), `encoding`, `marks_left`, `not_established`
   (`invisible-pixel-marks` first).

## §5 Tests

| test | protects | mutation |
|---|---|---|
| `framing_a_file_in_itself_is_stripping_it` (image) | `reframe(x, x) == strip(x)` | emit the original's IDAT |
| `a_re_encoded_png_keeps_its_colour_and_its_metadata` (image) | iCCP byte-identical; IDAT the new file's | the same |
| `a_palette_png_that_became_rgb_loses_what_spoke_of_the_palette` (image) | bKGD/sBIT dropped and said | keep them |
| `a_webp_framed_keeps_its_flags_true_and_its_size_counted` (image) | flags, RIFF size | skip the size |
| `a_jpeg_framed_takes_the_new_scans_and_keeps_its_camera_data` (image) | scans from the new file | — |
| `a_marked_png_is_restored_and_nothing_else_moves` | samples outside the rectangle identical through an independent `png` decode; iCCP identical; re-detection finds nothing | encode the unrestored raster |
| `a_marked_webp_is_restored_losslessly` | the same through `image-webp` | — |
| `nothing_restored_is_strip_byte_for_byte` | the no-op rule | always re-encode |
| `a_palette_png_stays_a_palette_when_it_can` / `…becomes_rgb_and_says_so` | colour type | always RGB |
| `c2pa_leaves_when_pixels_change` | D159 | keep C2PA under `reframe` |
| `a_jpeg_is_examined_and_not_yet_restored` | the E12-4 boundary | — |
| `an_animated_png_is_not_examined_and_says_so` | `NotExamined::Animated` | — |
| `the_proof_refuses_an_output_that_differs` (unit) | proof | drop the comparison |

## §7 Out of scope

JPEG and lossy WebP restoration (E12-4); surfaces (E12-5).
