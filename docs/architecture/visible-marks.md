# Visible marks — as built

`crates/wipemark-pixels` (E12-1): a generator's visible logo, found,
proved and taken off a **decoded raster**. The why — GeminiWatermarkTool
read in depth, the vendors, invisible marks, the measurements every
threshold here starts from — is [`docs/sdd/visible-marks.md`](../sdd/visible-marks.md);
the plan is [`docs/plan/E12-visible-marks.md`](../plan/E12-visible-marks.md).
This page says what exists.

## What the crate is, and is not

* Maths over a [`Raster`] — `width × height` pixels of `Rgb8`, `Rgba8`,
  `Rgb16` or `Rgba16`, every sample a `u16`, the **stored** values: no
  colour management, no EXIF rotation, alpha as it is (D157).
* **No codec and no file format.** Decoding a PNG into a raster, and
  writing one back, is `wipemark-picture`'s (E12-3). The dependency script
  keeps `wipemark-pixels → wipemark-core` the only workspace edge, and its
  external ones are `serde`, `serde_json`, `sha2` and `thiserror`.
* **No surface.** It hands up ids and numbers; the CLI and the MCP server
  word them (E12-5).
* `#![forbid(unsafe_code)]`; built at `opt-level = 3` in every profile
  (root `Cargo.toml`), because the search and the false-positive suite are
  orders of magnitude slower unoptimised.

## A mark is data

**The catalogue** is `manifests/marks.v1.json`, compiled in
(`include_str!`), schema 1. One **profile** per mark and opacity variant —
`gemini-sparkle-v1` and `gemini-sparkle-v2` ship; verification chooses
between them. A profile is: an id (a format: ASCII, never renamed, never
translated), vendor, product and mark (identifiers, shown beside a
finding, never inside a sentence), `observed`, `status`, the blend
(`encoded` with a logo colour; `linear-light` refused until a calibration
needs it, D152; `logo_map` refused in this version), `opaque_above`, its
opacity maps, its placement rows, its search, `detect.min_ncc` and the
three `verify` limits, and `source` for provenance.

**The opacity maps** are `.wma` files under `crates/wipemark-pixels/marks/`,
compiled in by `build.rs` (a table from file name to bytes) and pinned by
sha256 in the catalogue:

```
"WMA1" | u16 width | u16 height | u8 depth (8 or 16) | width × height samples, row-major
all little-endian; α = sample / (2^depth − 1)
```

GWT's four maps are depth 8, `sample = max(R, G, B)` of the PNG they came
from (`marks/README.md`); a calibrated map (E12-2) is depth 16.

**`Catalogue::shipped()`** parses once (`OnceLock`) and returns
`Result<&'static Catalogue, &'static CatalogueError>` — the choice the plan
offered: no `expect` in the product, and the shipped catalogue's validity
is a test (`the_shipped_catalogue_reads`). `Catalogue::parse(json, assets)`
is the same path for tests and the calibration tool. Everything is checked
before a profile exists — the schema, `deny_unknown_fields` on every
object, ids, every asset's pin (64 lower-case hex digits), hash, `.wma`
header and declared size, every row's map, a `when` that can match, a
`rect` whose size differs from its map only with `resample` and never in
aspect, thresholds in range, duplicates — and each failure is a
`CatalogueError` naming the profile and, for an asset, the map and the
`AssetProblem`.

> **State of the tree:** the four GWT maps are produced by
> `marks/gwt/extract.py` on a machine that can clone the reference; until
> then the manifest's pins read `pending: …` and `shipped()` is an `Err`.
> See `marks/README.md`.

## Propose → verify → choose → restore

`examine(raster, catalogue, options) -> Examination` is read-only;
`restore(raster, verified, options) -> Result<Restored, RestoreError>`;
`clean(raster, catalogue, options) -> PixelReport` is the whole pass.

1. **Propose** (`propose.rs`, the first proof). Luma is Rec. 601 over the
   stored values, in [0, 1]. NCC is `TM_CCOEFF_NORMED` against the map;
   the image side's mean and variance come from integral images (`f64`),
   and a window whose variance is below 10⁻¹⁰ per pixel is flat and scores
   0 rather than dividing rounding by rounding.
   * **Rows first**, every row whose `when` matches the size: the row's
     rectangle, then whole-pixel moves of ±3, then quarter-pixel moves of
     ±0.75 and size changes of ±0.5 in quarters. A move must beat the
     place it leaves by 10⁻⁴, so a tie keeps the row's own rectangle — the
     one that can be exact.
   * **The search**, only when no row reached `min_ncc`: sizes from the
     profile's range in steps of 4 over its corner box at a stride of
     `max(2, size/16)`, the five best candidates that do not overlap
     (IoU ≤ 0.3), a fine pass of ±4 in size and ±stride in position, then
     the same sub-pixel refinement.
   * **A map at a sub-pixel place and size** is the area-weighted mean of
     the map samples each pixel's footprint covers — an exact integral of
     the map read as constant per sample (the plan proposed a 4×4
     supersampling of bilinear samples; the integral is exact and makes
     "at its own size and an integer origin the template *is* the map"
     true by construction — `a_template_at_native_size_is_the_map`).
   * A proposal below `min_ncc` is not a finding and is dropped silently.
2. **Verify** (`verify.rs`, the second proof, D154). Over the template's
   rectangle and a one-pixel ring, with `|∇α|` (central differences)
   above 10⁻⁴ and no hole beside it:
   `E(img) = Σ |∇luma(img)| · |∇α|`. Sweep `k = 0, 0.02, …, 1.6` (`k = i/50`,
   so `k = 1` is exact), invert **unclamped** with `k·α`, measure `E`.
   Accept when `|k* − 1| ≤ gain`, `E(1)/E(0) ≤ edge_ratio`, and the share
   of samples the `k = 1` inverse puts more than one level out of range is
   at most `out_of_range`; otherwise `Refusal::Gain { k }`,
   `Edges { ratio }` or `OutOfRange { share }`, the first that failed, with
   every number in `Scores`. Before any of that: `Transparent` when the
   picture's alpha is below its maximum anywhere under the mark, and
   `Opaque { holes }` when every pixel of the mark is a hole. A picture
   with no edge at all where the map has one scores a ratio of 1.
   **`Verified`** has private fields, is built in `verify.rs` alone, and is
   the only thing `restore` takes (a `compile_fail` doctest).
3. **Choose.** Findings whose rectangles overlap (IoU > 0.3) compete:
   verified beats refused, then the lower edge ratio, within 0.01 a row
   beats the search, then the higher NCC. Losers are listed under the
   winner (`also_tried`), never as findings of their own — which is how V1
   beats V2 on a V1 picture.
4. **Restore** (`restore.rs`, which carries GWT's provenance header).
   Per pixel with `α ≥ 0.002`, per colour channel,
   `O = (I − α·L)/(1 − α)`, rounded half away from zero, clamped; a clamp
   beyond half a level is counted. `α ≥ opaque_above` is a **hole** (D155):
   untouched and counted. Alpha is never written. `exact` holds only for a
   lossless source, a row's own canonical map (no resample, integer
   origin), no hole and no clamp. A `Verified` from a raster of another
   size is `RestoreError::Elsewhere`.
5. **Again, once** (D165). After a restoration, `clean` examines the
   restored raster a second time; what verifies is restored, a refusal of
   the first pass seen again at the same place is not listed twice, and
   every finding carries its `pass`.

## The report

`PixelReport { found, restored, not_established }` and `to_json()` — one
line of ASCII JSON, field names a format:

```json
{"found":[{"profile":"…","vendor":"…","product":"…","pass":1,
  "rect":{"x":…,"y":…,"size":…},"pixels":{"x":…,"y":…,"width":…,"height":…},
  "placed":"row","row":1,"ncc":…,"verdict":"verified","refusal":null,
  "scores":{"gain":…,"edge_ratio":…,"out_of_range":…,"holes":0},
  "also_tried":[{"profile":"…","verified":false,"refusal":{"why":"gain","k":…}}]}],
 "restored":[{"profile":"…","rect":{…},"changed":…,"holes":0,"clamped":0,"exact":true}],
 "not_established":["invisible-pixel-marks","vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

**The third shelf** (D156): `not_established::ID` =
`invisible-pixel-marks`, "invisible marks in the picture's pixels — not
searched for, not removed", first, then core's three — on every report,
including one that found nothing. Its translations land with the first
surface that renders a picture report (E12-5). Nothing here says
"undetectable", "clean" or "AI-free" about a picture.

`PixelReport::marks_left()` is true when a mark was seen and is still
there — refused, or restored around holes — which is what a surface's
exit code reads.

## Thresholds

The shipped profiles carry the plan's starting values, from the spike's
measurements (SDD §1.6): `min_ncc` **0.70**, `gain` **0.06**, `edge_ratio`
**0.30**, `out_of_range` **0.01**, `opaque_above` **0.95**. They were not
re-measured when this was written — the container could compile but not
run — and `tests/measure.rs` prints the distributions (NCC, `k*`, ratio,
out-of-range for true marks, marks with ±3 noise, opaque look-alikes, the
0.72× variant and unmarked pictures) and the timings on a 2752×1536 raster
for whoever runs it:

```sh
cargo test -p wipemark-pixels --release --test measure -- --ignored --nocapture
```

Record what it prints in the step's report and here.

## Tests

* `tests/exact.rs` — composites on procedural pictures come back within a
  level, exact; alpha never written; a transparent region refused; holes
  never divided; a resampled row and a lossy source never exact; an
  unmarked picture never changed.
* `tests/verify.rs` — V1 told from V2; an opaque look-alike proposed and
  refused; the 0.72× variant refused with its gain; the inverse measured
  unclamped; a second mark found in the second pass, a baked-in one
  reported and not restored; refusals are values; the shelf; the JSON.
* `tests/false_positives.rs` — 2000 procedural negatives (textures,
  glyphs, flat, dark, bright, white corners, opaque and blurred sparkles,
  diamonds) × both synthetic profiles: nothing restored; the maxima are
  printed.
* `tests/assets.rs` — the shipped catalogue reads, every map is its PNG,
  every pin its file, a shipped mark comes back within a level; a tampered
  asset and a refused schema, on a synthetic profile.
* Unit tests in every module. No photograph and no vendor file is
  committed (Q-V8); the synthetic sparkle is an astroid drawn from its
  equation.

## Calibration (E12-2)

`calibrate.rs` and `examples/calibrate.rs` — how a vendor's mark becomes
a profile; a developer tool (D162), so no catalogue string and no row.
Plan: [`docs/plan/E12-2-calibration.md`](../plan/E12-2-calibration.md).

* `calibrate(&[Capture], &CalibrateOptions) -> Result<Calibration, CalibrationError>`
  for one output size: **locate** the support (each flat capture against
  a large box mean of itself, a pair against its clean twin; the bounding
  box of the mean deviation above `max(threshold, 0.2·peak)`, widened);
  fit the **background** under it as a quadratic per channel over a ring
  (a pair's twin is its own background); **regress** `I = a·B + c` per
  pixel and channel (`α = 1 − a`, `α·L = c`, `R²`); `L = Σc/Σα` where
  `α > 0.1`, its spread reported; `α` again by least squares with `L`
  fixed; the same in **linear light**; the **grey** captures, kept out of
  the fit whenever two other backgrounds remain, choose the model, and
  over 2 levels for both is `NotABlend`; **holes** at `opaque_above`, over
  5 % of the support `needs_reconstruction`. One background is
  `OneBackground` — black alone confounds `α` and `L` (SDD §1.2).
* `Calibration::wma()` writes depth 16; `Draft::to_json()` the row
  (`status: provisional`, one exact `rect` per size, the logo the mean of
  the sizes'); a linear-light mark is refused (`LinearLight`), as the
  catalogue would refuse it.
* `replay(catalogue, calibration, captures, …)` runs the profile over every
  capture: NCC, `k*`, ratio, verdict, and on flat captures and pairs the
  largest difference between the restored picture and the background.
* The tool: `cargo run --release -p wipemark-pixels --example calibrate --
  <captures dir> --out <dir>`, reading `captures.toml`
  (`examples/captures.example.toml`) and PNG (8 or 16 bit), JPEG or WebP;
  writing `<id>-<w>x<h>.wma`, `<id>.row.json`, `<id>.report.md` (the fit,
  the replay, and a false-positive pass over `WIPEMARK_FP_CORPUS` when it
  is set). Its decoders and `toml` are dev-dependencies: the library has
  no codec.
* The gate (`tests/calibrate.rs`): a synthetic vendor — a soft map, the
  tinted logo `(242, 246, 255)`, five black, five white and three grey
  captures with a vignette and ±1 noise — is recovered losslessly to
  1/255 (99th percentile; 2/255 at most) with `L` within a level, and
  through `image`'s JPEG encoder at quality 95 to 3/255 with `L` within
  two; a linear-light vendor is told by its grey captures.

## Picture files (E12-3)

`crates/wipemark-picture` — the one crate that decodes and encodes a
picture (`png`, `image-webp`, `zune-jpeg`), above `wipemark-image` (which
never decodes) and `wipemark-pixels` (which never reads a file). Plan:
[`docs/plan/E12-3-picture-files.md`](../plan/E12-3-picture-files.md).

* `decode(bytes, container)` — the stored raster: a palette and grey
  expanded to RGB(A) for the maths, 16 bits kept, alpha kept, no colour
  management, no rotation; what the file was (`Source`) remembered. An
  animated PNG or WebP is `Skip::Animated`.
* `encode_like(source, raster)` — PNG at the original's colour type and
  depth when the new values allow (a palette when every colour is in it,
  grey when every pixel is grey, sub-byte grey when every value has a
  code), otherwise RGB(A), said (`colour_changed`); interlace is not
  written, said (`interlace_dropped`). WebP: lossless `VP8L`.
* `inspect(bytes, options)` — both passes, read-only.
* `clean(bytes, options)` — metadata inspected on the original; the visible
  pass over the decoded raster; **nothing restored → `strip`'s output to
  the byte**; otherwise encode, `reframe` (C2PA leaves: both scopes take
  it, and its hard binding is to pixels that are gone), and **prove**:
  the output decodes to the restored raster, nothing outside the restored
  rectangles moved, nothing verifies on it — or `PictureError::Proof` and
  no output.
* What was not examined is a value: `Visible::NotExamined(Animated |
  Catalogue)`.
* **JPEG and lossy WebP (E12-4)** — the owner's answer to Q-V2/Q-V3
  (2026-10-04: "marks found are removed; re-encoding and the like do not
  matter"): decoded, restored, **re-encoded** — a JPEG with `image`'s
  `JpegEncoder` at `JPEG_QUALITY` 95 (4:4:4: the encoder has no other
  subsampling), a grey JPEG as grey; a lossy WebP as **lossless** `VP8L`
  (`from_lossy`). The metadata the scope keeps comes back through
  `reframe`. A lossy output cannot decode to the very samples, so its
  proof is a PSNR against the restored raster of at least `PSNR_FLOOR`
  34 dB, beside the other two checks. A CMYK JPEG is examined and **not
  restored** (`restorable: false`, every finding a mark left): its colour
  profile, which `reframe` keeps, speaks of inks the new file would not
  have. No coefficient codec and no block patch: the owner's answer made
  them unnecessary (D158 amended). Plan:
  [`docs/plan/E12-4-jpeg-and-lossy-webp.md`](../plan/E12-4-jpeg-and-lossy-webp.md).
* `PictureReport::to_json()` / `PictureInspection::to_json()` — the
  metadata JSON of E11, the pixel report of E12-1 (or why it did not run),
  the encoding, `marks_left`, and the picture's shelf with
  `invisible-pixel-marks` first.

## Not here yet

* **Surfaces**: the CLI and the MCP image tools, the catalogue strings —
  E12-5.
* **V2's small placements** — one exact row per Gemini output size, from
  GWT's `v2_small_config_from_dims`, ported into a test that generates the
  rows (`v2_rows_are_gwts_formula`). Not written: the formula could not be
  read from this container. Until it is, a V2 picture under 1025×1025 is
  found by the search, which is never exact.
* **Other vendors** (E12-6), **reconstruction** (E12-7), **the windows**
  (E12-8); restoring a `logo_map` or a linear-light mark.
